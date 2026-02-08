//! MCP tool handlers and method implementations.

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::diagnose;
use crate::operations;
use crate::paths::{LOGS_DIR, SPECS_DIR};
use crate::spec::{load_all_specs, resolve_spec, SpecStatus};
use crate::spec_group;

use super::protocol::{PROTOCOL_VERSION, SERVER_NAME, SERVER_VERSION};
use super::tools::{tool_chant_watch_start, tool_chant_watch_status, tool_chant_watch_stop};

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
                            "description": "Output text to append to spec body"
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
                "name": "chant_resume",
                "description": "Reset a failed spec to pending status so it can be reworked (deprecated: use 'chant_reset' instead)",
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

    match name {
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
        "chant_resume" => tool_chant_reset(arguments), // deprecated alias
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
    }
}

/// Find the project root by walking up from cwd looking for `.chant/` directory.
///
/// # Returns
///
/// - `Some(path)`: The directory containing `.chant/`
/// - `None`: No `.chant/` directory found in any parent
fn find_project_root() -> Option<PathBuf> {
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
fn mcp_ensure_initialized() -> Result<PathBuf, Value> {
    let project_root = find_project_root().ok_or_else(|| {
        json!({
            "content": [
                {
                    "type": "text",
                    "text": "Not in a chant project directory. Run `chant init` first or navigate to a directory containing `.chant/`."
                }
            ],
            "isError": true
        })
    })?;

    let specs_dir = project_root.join(SPECS_DIR);
    if !specs_dir.exists() {
        return Err(json!({
            "content": [
                {
                    "type": "text",
                    "text": "Chant not initialized. Run `chant init` first."
                }
            ],
            "isError": true
        }));
    }
    Ok(specs_dir)
}

fn tool_chant_spec_list(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let mut specs = load_all_specs(&specs_dir)?;
    specs.sort_by(|a, b| spec_group::compare_spec_ids(&a.id, &b.id));

    // Filter by status if provided
    if let Some(args) = arguments {
        if let Some(status_str) = args.get("status").and_then(|v| v.as_str()) {
            let filter_status = match status_str {
                "pending" => Some(SpecStatus::Pending),
                "in_progress" => Some(SpecStatus::InProgress),
                "completed" => Some(SpecStatus::Completed),
                "failed" => Some(SpecStatus::Failed),
                "cancelled" => Some(SpecStatus::Cancelled),
                _ => None,
            };

            if let Some(status) = filter_status {
                specs.retain(|s| s.frontmatter.status == status);
            }
        } else {
            // No status filter provided - filter out cancelled specs by default
            specs.retain(|s| s.frontmatter.status != SpecStatus::Cancelled);
        }
    } else {
        // No arguments provided - filter out cancelled specs by default
        specs.retain(|s| s.frontmatter.status != SpecStatus::Cancelled);
    }

    // Get limit (default 50)
    let limit = arguments
        .and_then(|a| a.get("limit"))
        .and_then(|v| v.as_u64())
        .unwrap_or(50) as usize;

    let total = specs.len();
    let limited_specs: Vec<_> = specs.into_iter().take(limit).collect();

    let specs_json: Vec<Value> = limited_specs
        .iter()
        .map(|s| {
            json!({
                "id": s.id,
                "title": s.title,
                "status": format!("{:?}", s.frontmatter.status).to_lowercase(),
                "type": s.frontmatter.r#type,
                "depends_on": s.frontmatter.depends_on,
                "labels": s.frontmatter.labels
            })
        })
        .collect();

    let response = json!({
        "specs": specs_json,
        "total": total,
        "limit": limit,
        "returned": limited_specs.len()
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

fn tool_chant_spec_get(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let id = arguments
        .and_then(|a| a.get("id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;

    let spec = match resolve_spec(&specs_dir, id) {
        Ok(s) => s,
        Err(e) => {
            return Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": e.to_string()
                    }
                ],
                "isError": true
            }));
        }
    };

    let spec_json = json!({
        "id": spec.id,
        "title": spec.title,
        "status": format!("{:?}", spec.frontmatter.status).to_lowercase(),
        "type": spec.frontmatter.r#type,
        "depends_on": spec.frontmatter.depends_on,
        "labels": spec.frontmatter.labels,
        "target_files": spec.frontmatter.target_files,
        "context": spec.frontmatter.context,
        "prompt": spec.frontmatter.prompt,
        "branch": spec.frontmatter.branch,
        "commits": spec.frontmatter.commits,
        "completed_at": spec.frontmatter.completed_at,
        "model": spec.frontmatter.model,
        "body": spec.body
    });

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&spec_json)?
            }
        ]
    }))
}

fn tool_chant_ready(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let mut specs = load_all_specs(&specs_dir)?;
    specs.sort_by(|a, b| spec_group::compare_spec_ids(&a.id, &b.id));

    // Filter to ready specs only
    let all_specs = specs.clone();
    specs.retain(|s| s.is_ready(&all_specs));
    // Filter out group specs - they are containers, not actionable work
    specs.retain(|s| s.frontmatter.r#type != "group");

    // Get limit (default 50)
    let limit = arguments
        .and_then(|a| a.get("limit"))
        .and_then(|v| v.as_u64())
        .unwrap_or(50) as usize;

    let total = specs.len();
    let limited_specs: Vec<_> = specs.into_iter().take(limit).collect();

    let specs_json: Vec<Value> = limited_specs
        .iter()
        .map(|s| {
            json!({
                "id": s.id,
                "title": s.title,
                "status": format!("{:?}", s.frontmatter.status).to_lowercase(),
                "type": s.frontmatter.r#type,
                "depends_on": s.frontmatter.depends_on,
                "labels": s.frontmatter.labels
            })
        })
        .collect();

    let response = json!({
        "specs": specs_json,
        "total": total,
        "limit": limit,
        "returned": limited_specs.len()
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

fn tool_chant_spec_update(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let args = arguments.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;

    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;

    let mut spec = match resolve_spec(&specs_dir, id) {
        Ok(s) => s,
        Err(e) => {
            return Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": e.to_string()
                    }
                ],
                "isError": true
            }));
        }
    };

    let spec_id = spec.id.clone();
    let spec_path = specs_dir.join(format!("{}.md", spec.id));

    // Parse status if provided
    let status = if let Some(status_str) = args.get("status").and_then(|v| v.as_str()) {
        let new_status = match status_str {
            "pending" => SpecStatus::Pending,
            "in_progress" => SpecStatus::InProgress,
            "completed" => SpecStatus::Completed,
            "failed" => SpecStatus::Failed,
            _ => {
                return Ok(json!({
                    "content": [
                        {
                            "type": "text",
                            "text": format!("Invalid status: {}. Must be one of: pending, in_progress, completed, failed", status_str)
                        }
                    ],
                    "isError": true
                }));
            }
        };
        Some(new_status)
    } else {
        None
    };

    // Parse other fields
    let depends_on = args
        .get("depends_on")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        });

    let labels = args.get("labels").and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    });

    let target_files = args
        .get("target_files")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        });

    let model = args.get("model").and_then(|v| v.as_str()).map(String::from);

    let output = args
        .get("output")
        .and_then(|v| v.as_str())
        .map(String::from);

    // Use operations module for update
    let options = crate::operations::update::UpdateOptions {
        status,
        depends_on,
        labels,
        target_files,
        model,
        output,
        force: true, // MCP updates bypass validation for backwards compatibility
    };

    match crate::operations::update::update_spec(&mut spec, &spec_path, options) {
        Ok(_) => Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": format!("Updated spec: {}", spec_id)
                }
            ]
        })),
        Err(e) => Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": format!("Failed to update spec: {}", e)
                }
            ],
            "isError": true
        })),
    }
}

fn tool_chant_status(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let specs = load_all_specs(&specs_dir)?;

    // Parse options
    let brief = arguments
        .and_then(|a| a.get("brief"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let include_activity = arguments
        .and_then(|a| a.get("include_activity"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Count by status
    let mut pending = 0;
    let mut in_progress = 0;
    let mut completed = 0;
    let mut failed = 0;
    let mut blocked = 0;
    let mut cancelled = 0;
    let mut needs_attention = 0;

    for spec in &specs {
        match spec.frontmatter.status {
            SpecStatus::Pending => pending += 1,
            SpecStatus::InProgress => in_progress += 1,
            SpecStatus::Paused => in_progress += 1, // Count paused as in_progress for summary
            SpecStatus::Completed => completed += 1,
            SpecStatus::Failed => failed += 1,
            SpecStatus::Ready => pending += 1, // Ready is computed, treat as pending
            SpecStatus::Blocked => blocked += 1,
            SpecStatus::Cancelled => cancelled += 1,
            SpecStatus::NeedsAttention => needs_attention += 1,
        }
    }

    // Brief output mode
    if brief {
        let mut parts = vec![];
        if pending > 0 {
            parts.push(format!("{} pending", pending));
        }
        if in_progress > 0 {
            parts.push(format!("{} in_progress", in_progress));
        }
        if completed > 0 {
            parts.push(format!("{} completed", completed));
        }
        if failed > 0 {
            parts.push(format!("{} failed", failed));
        }
        if blocked > 0 {
            parts.push(format!("{} blocked", blocked));
        }
        if cancelled > 0 {
            parts.push(format!("{} cancelled", cancelled));
        }
        if needs_attention > 0 {
            parts.push(format!("{} needs_attention", needs_attention));
        }
        let brief_text = if parts.is_empty() {
            "No specs".to_string()
        } else {
            parts.join(" | ")
        };
        return Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": brief_text
                }
            ]
        }));
    }

    // Build status response
    let mut status_json = json!({
        "total": specs.len(),
        "pending": pending,
        "in_progress": in_progress,
        "completed": completed,
        "failed": failed,
        "blocked": blocked,
        "cancelled": cancelled,
        "needs_attention": needs_attention
    });

    // Include activity info for in_progress specs
    if include_activity {
        let logs_dir = PathBuf::from(LOGS_DIR);
        let mut activity: Vec<Value> = vec![];

        for spec in &specs {
            if spec.frontmatter.status != SpecStatus::InProgress {
                continue;
            }

            let spec_path = specs_dir.join(format!("{}.md", spec.id));
            let log_path = logs_dir.join(format!("{}.log", spec.id));

            // Get spec file modification time
            let spec_mtime = std::fs::metadata(&spec_path)
                .and_then(|m| m.modified())
                .ok()
                .map(|t| {
                    chrono::DateTime::<chrono::Local>::from(t)
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                });

            // Get log file modification time (indicates last agent activity)
            let log_mtime = std::fs::metadata(&log_path)
                .and_then(|m| m.modified())
                .ok()
                .map(|t| {
                    chrono::DateTime::<chrono::Local>::from(t)
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                });

            // Check if log file exists
            let has_log = log_path.exists();

            activity.push(json!({
                "id": spec.id,
                "title": spec.title,
                "spec_modified": spec_mtime,
                "log_modified": log_mtime,
                "has_log": has_log
            }));
        }

        status_json["in_progress_activity"] = json!(activity);
    }

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&status_json)?
            }
        ]
    }))
}

fn tool_chant_log(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let args = arguments.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;

    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;

    let lines = args
        .get("lines")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);
    let byte_offset = args.get("offset").and_then(|v| v.as_u64());
    let since = args.get("since").and_then(|v| v.as_str());

    // Resolve spec to get full ID
    let spec = match resolve_spec(&specs_dir, id) {
        Ok(s) => s,
        Err(e) => {
            return Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": e.to_string()
                    }
                ],
                "isError": true
            }));
        }
    };

    let logs_dir = PathBuf::from(LOGS_DIR);
    let log_path = logs_dir.join(format!("{}.log", spec.id));

    if !log_path.exists() {
        return Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": format!("No log file found for spec '{}'. Logs are created when a spec is executed with `chant work`.", spec.id)
                }
            ],
            "isError": true
        }));
    }

    // Read log file
    let content = std::fs::read_to_string(&log_path)?;
    let file_byte_len = content.len() as u64;

    // Filter by offset if provided
    let content_after_offset = if let Some(offset) = byte_offset {
        if offset >= file_byte_len {
            // Offset is at or beyond end of file
            String::new()
        } else {
            content[(offset as usize)..].to_string()
        }
    } else {
        content.clone()
    };

    // Filter by timestamp if provided
    let content_after_since = if let Some(since_ts) = since {
        if let Ok(since_time) = chrono::DateTime::parse_from_rfc3339(since_ts) {
            content_after_offset
                .lines()
                .filter(|line| {
                    // Try to parse timestamp from line start
                    // Assumes log format: YYYY-MM-DDTHH:MM:SS.sssZ ...
                    if line.len() >= 24 {
                        if let Ok(line_time) = chrono::DateTime::parse_from_rfc3339(&line[..24]) {
                            return line_time > since_time;
                        }
                    }
                    true // Include lines without parseable timestamps
                })
                .collect::<Vec<&str>>()
                .join("\n")
        } else {
            content_after_offset
        }
    } else {
        content_after_offset
    };

    // Apply lines limit
    let all_lines: Vec<&str> = content_after_since.lines().collect();
    let lines_limit = lines.unwrap_or(100);
    let start = if all_lines.len() > lines_limit {
        all_lines.len() - lines_limit
    } else {
        0
    };
    let log_output = all_lines[start..].join("\n");

    // Calculate new byte offset
    let new_byte_offset = if byte_offset.is_some() {
        file_byte_len
    } else {
        content.len() as u64
    };

    let has_more = all_lines.len() > lines_limit;
    let line_count = all_lines[start..].len();

    let response = json!({
        "content": log_output,
        "byte_offset": new_byte_offset,
        "line_count": line_count,
        "has_more": has_more
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

fn tool_chant_search(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let args = arguments.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;

    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: query"))?
        .to_lowercase();

    let status_filter = args.get("status").and_then(|v| v.as_str());

    let mut specs = load_all_specs(&specs_dir)?;

    // Filter by query (case-insensitive search in title and body)
    specs.retain(|s| {
        let title_match = s
            .title
            .as_ref()
            .map(|t| t.to_lowercase().contains(&query))
            .unwrap_or(false);
        title_match || s.body.to_lowercase().contains(&query)
    });

    // Filter by status if provided
    if let Some(status_str) = status_filter {
        let filter_status = match status_str {
            "pending" => Some(SpecStatus::Pending),
            "in_progress" => Some(SpecStatus::InProgress),
            "completed" => Some(SpecStatus::Completed),
            "failed" => Some(SpecStatus::Failed),
            "blocked" => Some(SpecStatus::Blocked),
            "cancelled" => Some(SpecStatus::Cancelled),
            _ => None,
        };

        if let Some(status) = filter_status {
            specs.retain(|s| s.frontmatter.status == status);
        }
    }

    specs.sort_by(|a, b| spec_group::compare_spec_ids(&a.id, &b.id));

    let specs_json: Vec<Value> = specs
        .iter()
        .map(|s| {
            json!({
                "id": s.id,
                "title": s.title,
                "status": format!("{:?}", s.frontmatter.status).to_lowercase(),
                "type": s.frontmatter.r#type
            })
        })
        .collect();

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&specs_json)?
            }
        ]
    }))
}

fn tool_chant_diagnose(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let args = arguments.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;

    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;

    // Resolve spec to get full ID
    let spec = match resolve_spec(&specs_dir, id) {
        Ok(s) => s,
        Err(e) => {
            return Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": e.to_string()
                    }
                ],
                "isError": true
            }));
        }
    };

    // Run diagnostics
    let report = match diagnose::diagnose_spec(&spec.id) {
        Ok(r) => r,
        Err(e) => {
            return Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": format!("Failed to diagnose spec: {}", e)
                    }
                ],
                "isError": true
            }));
        }
    };

    // Format report as JSON
    let checks_json: Vec<Value> = report
        .checks
        .iter()
        .map(|c| {
            json!({
                "name": c.name,
                "passed": c.passed,
                "details": c.details
            })
        })
        .collect();

    let report_json = json!({
        "spec_id": report.spec_id,
        "status": format!("{:?}", report.status).to_lowercase(),
        "location": report.location,
        "checks": checks_json,
        "diagnosis": report.diagnosis,
        "suggestion": report.suggestion
    });

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&report_json)?
            }
        ]
    }))
}

fn tool_chant_lint(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let spec_id = arguments
        .and_then(|args| args.get("id"))
        .and_then(|v| v.as_str());

    use crate::config::Config;
    use crate::score::traffic_light;
    use crate::scoring::{calculate_spec_score, TrafficLight};
    use crate::spec::Spec;

    // Load config for scoring
    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            return Ok(json!({
                "content": [{ "type": "text", "text": format!("Failed to load config: {}", e) }],
                "isError": true
            }));
        }
    };

    // Load all specs (needed for isolation scoring)
    let all_specs = load_all_specs(&specs_dir)?;

    // Collect specs to check
    let specs_to_check: Vec<Spec> = if let Some(id) = spec_id {
        match resolve_spec(&specs_dir, id) {
            Ok(spec) => vec![spec],
            Err(e) => {
                return Ok(json!({
                    "content": [{ "type": "text", "text": e.to_string() }],
                    "isError": true
                }));
            }
        }
    } else {
        all_specs.clone()
    };

    let mut results: Vec<Value> = Vec::new();
    let mut red_count = 0;
    let mut yellow_count = 0;
    let mut green_count = 0;

    // Run full quality assessment on each spec (same as chant work)
    for spec in &specs_to_check {
        let score = calculate_spec_score(spec, &all_specs, &config);
        let suggestions = traffic_light::generate_suggestions(&score);

        let traffic_light_str = match score.traffic_light {
            TrafficLight::Ready => {
                green_count += 1;
                "green"
            }
            TrafficLight::Review => {
                yellow_count += 1;
                "yellow"
            }
            TrafficLight::Refine => {
                red_count += 1;
                "red"
            }
        };

        results.push(json!({
            "id": spec.id,
            "title": spec.title,
            "traffic_light": traffic_light_str,
            "complexity": score.complexity.to_string(),
            "confidence": score.confidence.to_string(),
            "splittability": score.splittability.to_string(),
            "ac_quality": score.ac_quality.to_string(),
            "isolation": score.isolation.map(|i| i.to_string()),
            "suggestions": suggestions
        }));
    }

    let summary = json!({
        "specs_checked": specs_to_check.len(),
        "red": red_count,
        "yellow": yellow_count,
        "green": green_count,
        "results": results
    });

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&summary)?
            }
        ]
    }))
}

fn tool_chant_add(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let args = arguments.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;

    let description = args
        .get("description")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: description"))?;

    let prompt = args.get("prompt").and_then(|v| v.as_str());

    // Load config for derivation
    let config = match crate::config::Config::load() {
        Ok(c) => c,
        Err(e) => {
            return Ok(json!({
                "content": [{ "type": "text", "text": format!("Failed to load config: {}", e) }],
                "isError": true
            }));
        }
    };

    // Use operations module for spec creation
    let options = crate::operations::create::CreateOptions {
        prompt: prompt.map(String::from),
        needs_approval: false,
        auto_commit: false, // MCP doesn't auto-commit
    };

    let (spec, _filepath) = match crate::operations::create::create_spec(
        description,
        &specs_dir,
        &config,
        options,
    ) {
        Ok(result) => result,
        Err(e) => {
            return Ok(json!({
                "content": [{ "type": "text", "text": format!("Failed to create spec: {}", e) }],
                "isError": true
            }));
        }
    };

    let response_text = format!("Created spec: {}", spec.id);

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": response_text
            }
        ]
    }))
}

fn tool_chant_finalize(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let args = arguments.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;

    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;

    let mut spec = match resolve_spec(&specs_dir, id) {
        Ok(s) => s,
        Err(e) => {
            return Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": e.to_string()
                    }
                ],
                "isError": true
            }));
        }
    };

    let spec_id = spec.id.clone();

    // Check if spec is in valid state for finalization
    match spec.frontmatter.status {
        SpecStatus::Completed | SpecStatus::InProgress | SpecStatus::Failed => {
            // Valid for finalization
        }
        _ => {
            return Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": format!("Spec '{}' must be in_progress, completed, or failed to finalize. Current status: {:?}", spec_id, spec.frontmatter.status)
                    }
                ],
                "isError": true
            }));
        }
    }

    // Check for unchecked acceptance criteria
    let unchecked = spec.count_unchecked_checkboxes();
    if unchecked > 0 {
        return Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": format!("Spec '{}' has {} unchecked acceptance criteria. All criteria must be checked before finalization.", spec_id, unchecked)
                }
            ],
            "isError": true
        }));
    }

    // Load config and all specs for finalization
    let config = match crate::config::Config::load() {
        Ok(c) => c,
        Err(e) => {
            return Ok(json!({
                "content": [{ "type": "text", "text": format!("Failed to load config: {}", e) }],
                "isError": true
            }));
        }
    };

    let all_specs = match load_all_specs(&specs_dir) {
        Ok(specs) => specs,
        Err(e) => {
            return Ok(json!({
                "content": [{ "type": "text", "text": format!("Failed to load specs: {}", e) }],
                "isError": true
            }));
        }
    };

    // Use operations module for finalization with full validation
    let spec_repo = crate::repository::spec_repository::FileSpecRepository::new(specs_dir.clone());
    let options = crate::operations::finalize::FinalizeOptions {
        allow_no_commits: false,
        commits: None, // Auto-detect commits
    };

    match crate::operations::finalize::finalize_spec(
        &mut spec, &spec_repo, &config, &all_specs, options,
    ) {
        Ok(_) => Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": format!("Finalized spec: {}", spec_id)
                }
            ]
        })),
        Err(e) => Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": format!("Failed to finalize spec: {}", e)
                }
            ],
            "isError": true
        })),
    }
}

fn tool_chant_reset(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let args = arguments.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;

    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;

    let mut spec = match resolve_spec(&specs_dir, id) {
        Ok(s) => s,
        Err(e) => {
            return Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": e.to_string()
                    }
                ],
                "isError": true
            }));
        }
    };

    let spec_id = spec.id.clone();
    let spec_path = specs_dir.join(format!("{}.md", spec.id));

    // Use operations module for reset
    let options = crate::operations::reset::ResetOptions::default();

    match crate::operations::reset::reset_spec(&mut spec, &spec_path, options) {
        Ok(_) => Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": format!("Reset spec '{}' to pending", spec_id)
                }
            ]
        })),
        Err(e) => Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": format!("Failed to reset spec: {}", e)
                }
            ],
            "isError": true
        })),
    }
}

fn tool_chant_cancel(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let args = arguments.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;

    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;

    let options = operations::CancelOptions::default();

    match operations::cancel_spec(&specs_dir, id, &options) {
        Ok(spec) => Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": format!("Cancelled spec: {}", spec.id)
                }
            ]
        })),
        Err(e) => Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": e.to_string()
                }
            ],
            "isError": true
        })),
    }
}

fn tool_chant_archive(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let args = arguments.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;

    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;

    let options = operations::ArchiveOptions::default();

    match operations::archive_spec(&specs_dir, id, &options) {
        Ok(dest_path) => Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": format!("Archived spec: {} -> {}", id, dest_path.display())
                }
            ]
        })),
        Err(e) => Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": e.to_string()
                }
            ],
            "isError": true
        })),
    }
}

fn tool_chant_verify(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let args = arguments.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;

    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;

    let spec = match resolve_spec(&specs_dir, id) {
        Ok(s) => s,
        Err(e) => {
            return Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": e.to_string()
                    }
                ],
                "isError": true
            }));
        }
    };

    let spec_id = spec.id.clone();

    // Count checked and unchecked criteria using operations layer
    let unchecked_count = spec.count_unchecked_checkboxes();

    // Find total checkboxes in Acceptance Criteria section
    let total_count: usize = {
        let acceptance_criteria_marker = "## Acceptance Criteria";
        let mut in_ac_section = false;
        let mut in_code_fence = false;
        let mut count = 0;

        for line in spec.body.lines() {
            let trimmed = line.trim_start();

            if trimmed.starts_with("```") {
                in_code_fence = !in_code_fence;
                continue;
            }

            if !in_code_fence && trimmed.starts_with(acceptance_criteria_marker) {
                in_ac_section = true;
                continue;
            }

            if in_ac_section && trimmed.starts_with("## ") && !in_code_fence {
                break;
            }

            if in_ac_section
                && !in_code_fence
                && (trimmed.starts_with("- [x]") || trimmed.starts_with("- [ ]"))
            {
                count += 1;
            }
        }

        count
    };

    let checked_count = total_count.saturating_sub(unchecked_count);
    let verified = unchecked_count == 0 && total_count > 0;

    // Extract unchecked items
    let unchecked_items = if unchecked_count > 0 {
        let acceptance_criteria_marker = "## Acceptance Criteria";
        let mut in_ac_section = false;
        let mut in_code_fence = false;
        let mut items = Vec::new();

        for line in spec.body.lines() {
            let trimmed = line.trim_start();

            if trimmed.starts_with("```") {
                in_code_fence = !in_code_fence;
                continue;
            }

            if !in_code_fence && trimmed.starts_with(acceptance_criteria_marker) {
                in_ac_section = true;
                continue;
            }

            if in_ac_section && trimmed.starts_with("## ") && !in_code_fence {
                break;
            }

            if in_ac_section && !in_code_fence && trimmed.starts_with("- [ ]") {
                items.push(trimmed.to_string());
            }
        }

        items
    } else {
        Vec::new()
    };

    let verification_notes = if total_count == 0 {
        "No acceptance criteria found".to_string()
    } else if verified {
        "All acceptance criteria met".to_string()
    } else {
        format!("{} criteria not yet checked", unchecked_count)
    };

    let result = json!({
        "spec_id": spec_id,
        "verified": verified,
        "criteria": {
            "total": total_count,
            "checked": checked_count,
            "unchecked": unchecked_count
        },
        "unchecked_items": unchecked_items,
        "verification_notes": verification_notes
    });

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&result)?
            }
        ]
    }))
}

fn tool_chant_work_start(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let args = arguments.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;

    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;

    let chain = args.get("chain").and_then(|v| v.as_bool()).unwrap_or(false);
    let parallel = args.get("parallel").and_then(|v| v.as_u64());
    let skip_criteria = args
        .get("skip_criteria")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Resolve spec to get full ID
    let spec = match resolve_spec(&specs_dir, id) {
        Ok(s) => s,
        Err(e) => {
            return Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": e.to_string()
                    }
                ],
                "isError": true
            }));
        }
    };

    let spec_id = spec.id.clone();

    // Pre-validate spec quality unless skip_criteria is true
    // This prevents silent failures in non-interactive mode
    if !skip_criteria {
        use crate::config::Config;
        use crate::scoring::{calculate_spec_score, TrafficLight};

        let config = match Config::load() {
            Ok(c) => c,
            Err(e) => {
                return Ok(json!({
                    "content": [
                        {
                            "type": "text",
                            "text": format!("Failed to load config: {}", e)
                        }
                    ],
                    "isError": true
                }));
            }
        };

        let all_specs = match load_all_specs(&specs_dir) {
            Ok(specs) => specs,
            Err(e) => {
                return Ok(json!({
                    "content": [
                        {
                            "type": "text",
                            "text": format!("Failed to load specs: {}", e)
                        }
                    ],
                    "isError": true
                }));
            }
        };

        let quality_score = calculate_spec_score(&spec, &all_specs, &config);

        if quality_score.traffic_light == TrafficLight::Refine {
            use crate::score::traffic_light;

            let suggestions = traffic_light::generate_suggestions(&quality_score);
            let mut error_message = format!(
                "Spec '{}' has quality issues (status: Red/Refine) that may cause problems:\n\n",
                spec_id
            );

            error_message.push_str("Quality Assessment:\n");
            error_message.push_str(&format!("  Complexity:    {}\n", quality_score.complexity));
            error_message.push_str(&format!("  Confidence:    {}\n", quality_score.confidence));
            error_message.push_str(&format!(
                "  Splittability: {}\n",
                quality_score.splittability
            ));
            error_message.push_str(&format!("  AC Quality:    {}\n", quality_score.ac_quality));
            if let Some(iso) = quality_score.isolation {
                error_message.push_str(&format!("  Isolation:     {}\n", iso));
            }

            if !suggestions.is_empty() {
                error_message.push_str("\nSuggestions:\n");
                for suggestion in &suggestions {
                    error_message.push_str(&format!("  • {}\n", suggestion));
                }
            }

            error_message.push_str("\nTo bypass quality checks, use skip_criteria: true\n");

            return Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": error_message
                    }
                ],
                "isError": true
            }));
        }
    }

    // Update status to in_progress after quality validation passes (matches CLI behavior)
    use crate::spec::TransitionBuilder;
    let mut spec = spec;
    let spec_path = specs_dir.join(format!("{}.md", spec_id));

    TransitionBuilder::new(&mut spec)
        .to(SpecStatus::InProgress)
        .map_err(|e| anyhow::anyhow!("Failed to transition spec to in_progress: {}", e))?;

    spec.save(&spec_path)?;

    // Build command based on mode
    let mut cmd = Command::new("chant");
    cmd.arg("work");

    if skip_criteria {
        cmd.arg("--skip-criteria");
    }

    let mode = if let Some(p) = parallel {
        cmd.arg("--parallel").arg(p.to_string());
        format!("parallel({})", p)
    } else if chain {
        cmd.arg("--chain").arg(&spec_id);
        "chain".to_string()
    } else {
        cmd.arg(&spec_id);
        "single".to_string()
    };

    // Spawn as background process
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let mut child = cmd.spawn().context("Failed to spawn chant work process")?;

    let pid = child.id();
    let started_at = chrono::Local::now().to_rfc3339();
    let process_id = format!("{}-{}", spec_id, pid);

    // Spawn a thread to reap the process when it exits (prevents zombies)
    std::thread::spawn(move || {
        let _ = child.wait();
    });

    // Store process info
    let project_root =
        find_project_root().ok_or_else(|| anyhow::anyhow!("Project root not found"))?;
    let processes_dir = project_root.join(".chant/processes");
    std::fs::create_dir_all(&processes_dir)?;

    let process_info = json!({
        "process_id": process_id,
        "spec_id": spec_id,
        "pid": pid,
        "started_at": started_at,
        "mode": mode
    });

    let process_file = processes_dir.join(format!("{}.json", process_id));
    std::fs::write(&process_file, serde_json::to_string_pretty(&process_info)?)?;

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&process_info)?
            }
        ]
    }))
}

fn tool_chant_work_list(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let include_completed = arguments
        .and_then(|a| a.get("include_completed"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Use PID files to determine running processes (reliable source of truth)
    let active_pids = crate::pid::list_active_pids()?;

    // Load all specs to get metadata
    let all_specs = load_all_specs(&specs_dir)?;
    let spec_map: std::collections::HashMap<String, &crate::spec::Spec> =
        all_specs.iter().map(|s| (s.id.clone(), s)).collect();

    let mut processes: Vec<Value> = Vec::new();
    let mut running = 0;
    let mut stale_count = 0;

    let logs_dir = PathBuf::from(LOGS_DIR);

    // Report processes with active PIDs
    for (spec_id, pid, is_running) in &active_pids {
        if !is_running {
            stale_count += 1;
            if !include_completed {
                continue;
            }
        } else {
            running += 1;
        }

        let spec = spec_map.get(spec_id);
        let title = spec.and_then(|s| s.title.as_deref());
        let branch = spec.and_then(|s| s.frontmatter.branch.as_deref());

        let log_path = logs_dir.join(format!("{}.log", spec_id));
        let log_mtime = if log_path.exists() {
            std::fs::metadata(&log_path)
                .and_then(|m| m.modified())
                .ok()
                .map(|t| {
                    chrono::DateTime::<chrono::Local>::from(t)
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                })
        } else {
            None
        };

        processes.push(json!({
            "spec_id": spec_id,
            "title": title,
            "pid": pid,
            "status": if *is_running { "running" } else { "stale" },
            "log_modified": log_mtime,
            "branch": branch
        }));
    }

    let summary = json!({
        "running": running,
        "stale": stale_count
    });

    let response = json!({
        "processes": processes,
        "summary": summary
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

fn tool_chant_pause(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let args = arguments.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;

    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;

    // Resolve spec to get full ID
    let mut spec = match resolve_spec(&specs_dir, id) {
        Ok(s) => s,
        Err(e) => {
            return Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": e.to_string()
                    }
                ],
                "isError": true
            }));
        }
    };

    let spec_id = spec.id.clone();
    let spec_path = specs_dir.join(format!("{}.md", spec_id));

    // Use operations layer (MCP always forces pause)
    let options = crate::operations::PauseOptions { force: true };
    crate::operations::pause_spec(&mut spec, &spec_path, options)?;

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": format!("Successfully paused work for spec '{}'", spec_id)
            }
        ]
    }))
}

fn tool_chant_takeover(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let args = arguments.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;

    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;

    let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);

    // Resolve spec to get full ID
    let spec = match resolve_spec(&specs_dir, id) {
        Ok(s) => s,
        Err(e) => {
            return Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": e.to_string()
                    }
                ],
                "isError": true
            }));
        }
    };

    // Execute takeover
    match crate::takeover::cmd_takeover(&spec.id, force) {
        Ok(result) => {
            let response = json!({
                "spec_id": result.spec_id,
                "analysis": result.analysis,
                "log_tail": result.log_tail,
                "suggestion": result.suggestion,
                "worktree_path": result.worktree_path
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
        Err(e) => Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": format!("Failed to take over spec '{}': {}", spec.id, e)
                }
            ],
            "isError": true
        })),
    }
}

fn tool_chant_split(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let args = arguments.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;

    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;

    let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);
    let recursive = args
        .get("recursive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let max_depth = args.get("max_depth").and_then(|v| v.as_u64());

    // Resolve spec to validate it exists
    let spec = match resolve_spec(&specs_dir, id) {
        Ok(s) => s,
        Err(e) => {
            return Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": e.to_string()
                    }
                ],
                "isError": true
            }));
        }
    };

    let spec_id = spec.id.clone();

    // Check if spec is in valid state for splitting
    match spec.frontmatter.status {
        SpecStatus::Pending => {
            // Valid for splitting
        }
        _ => {
            return Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": format!("Spec '{}' must be in pending status to split. Current status: {:?}", spec_id, spec.frontmatter.status)
                    }
                ],
                "isError": true
            }));
        }
    }

    // Build command
    let mut cmd = Command::new("chant");
    cmd.arg("split");
    cmd.arg(&spec_id);

    if force {
        cmd.arg("--force");
    }
    if recursive {
        cmd.arg("--recursive");
    }
    if let Some(depth) = max_depth {
        cmd.arg("--max-depth").arg(depth.to_string());
    }

    // Spawn as background process
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let mut child = cmd.spawn().context("Failed to spawn chant split process")?;

    let pid = child.id();
    let started_at = chrono::Local::now().to_rfc3339();
    let process_id = format!("split-{}-{}", spec_id, pid);

    // Spawn a thread to reap the process when it exits (prevents zombies)
    std::thread::spawn(move || {
        let _ = child.wait();
    });

    // Store process info
    let project_root =
        find_project_root().ok_or_else(|| anyhow::anyhow!("Project root not found"))?;
    let processes_dir = project_root.join(".chant/processes");
    std::fs::create_dir_all(&processes_dir)?;

    let process_info = json!({
        "process_id": process_id,
        "spec_id": spec_id,
        "pid": pid,
        "started_at": started_at,
        "mode": "split",
        "options": {
            "force": force,
            "recursive": recursive,
            "max_depth": max_depth
        }
    });

    let process_file = processes_dir.join(format!("{}.json", process_id));
    std::fs::write(&process_file, serde_json::to_string_pretty(&process_info)?)?;

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&process_info)?
            }
        ]
    }))
}
