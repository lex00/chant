//! Model Context Protocol (MCP) server implementation.
//!
//! # Doc Audit
//! - audited: 2026-01-25
//! - docs: reference/mcp.md
//! - ignore: false

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use chant::diagnose;
use chant::id;
use chant::paths::{ARCHIVE_DIR, LOGS_DIR, SPECS_DIR};
use chant::spec::{load_all_specs, resolve_spec, SpecStatus};

/// JSON-RPC 2.0 Request
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    #[serde(default)]
    params: Option<Value>,
    id: Option<Value>,
}

/// JSON-RPC 2.0 Response
///
/// Represents a JSON-RPC 2.0 response message. Either `result` or `error` will be present,
/// but not both.
///
/// # Success Response
///
/// When the request succeeds, `result` contains the response data and `error` is `None`.
///
/// # Error Response
///
/// When the request fails, `error` contains error details and `result` is `None`.
///
/// # Fields
///
/// - `jsonrpc`: Version string, always `"2.0"`
/// - `result`: Success data (tool result or handler response)
/// - `error`: Error details if request failed
/// - `id`: Request ID from the original request (for correlation)
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
    id: Value,
}

/// JSON-RPC 2.0 Error
///
/// Represents an error in a JSON-RPC response.
///
/// # Error Codes
///
/// - `-32700`: Parse error (invalid JSON)
/// - `-32600`: Invalid JSON-RPC version (jsonrpc != "2.0")
/// - `-32603`: Server error (internal handler error)
///
/// # Fields
///
/// - `code`: JSON-RPC error code (negative integer)
/// - `message`: Human-readable error description
/// - `data`: Optional additional error context
#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

impl JsonRpcResponse {
    /// Create a successful response.
    ///
    /// # Arguments
    ///
    /// - `id`: Request ID to echo back
    /// - `result`: Response data (typically a JSON object)
    ///
    /// # Returns
    ///
    /// A response with `result` set and `error` as `None`.
    fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    /// Create an error response.
    ///
    /// # Arguments
    ///
    /// - `id`: Request ID to echo back
    /// - `code`: JSON-RPC error code (negative integer)
    ///   - `-32700`: Parse error
    ///   - `-32600`: Invalid JSON-RPC version
    ///   - `-32603`: Server error
    /// - `message`: Human-readable error description
    ///
    /// # Returns
    ///
    /// A response with `error` set and `result` as `None`.
    fn error(id: Value, code: i32, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.to_string(),
                data: None,
            }),
            id,
        }
    }
}

/// MCP Server info
const SERVER_NAME: &str = "chant";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const PROTOCOL_VERSION: &str = "2024-11-05";

/// Run the MCP server, reading from stdin and writing to stdout.
pub fn run_server() -> Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let reader = BufReader::new(stdin.lock());

    for line in reader.lines() {
        let line = line.context("Failed to read from stdin")?;

        if line.trim().is_empty() {
            continue;
        }

        let response = handle_request(&line);

        if let Some(resp) = response {
            let output = serde_json::to_string(&resp)?;
            writeln!(stdout, "{}", output)?;
            stdout.flush()?;
        }
    }

    Ok(())
}

/// Handle a single JSON-RPC request line.
///
/// # Request Processing
///
/// 1. Parse JSON-RPC 2.0 request from the line
/// 2. Validate `jsonrpc` field is `"2.0"`
/// 3. Dispatch to appropriate handler based on `method`
/// 4. Return response or `None` for notifications
///
/// # Error Handling
///
/// - **Parse Error (-32700)**: JSON is invalid or malformed
/// - **Invalid Version (-32600)**: `jsonrpc` field is not `"2.0"`
/// - **Server Error (-32603)**: Handler function returns `Err`
/// - **No Response**: Notifications (requests without `id`) are handled silently
///
/// # Returns
///
/// - `Some(response)`: For requests (with `id`)
/// - `None`: For notifications (without `id`)
fn handle_request(line: &str) -> Option<JsonRpcResponse> {
    let request: JsonRpcRequest = match serde_json::from_str(line) {
        Ok(req) => req,
        Err(e) => {
            return Some(JsonRpcResponse::error(
                Value::Null,
                -32700,
                &format!("Parse error: {}", e),
            ));
        }
    };

    // Validate jsonrpc version
    if request.jsonrpc != "2.0" {
        return Some(JsonRpcResponse::error(
            request.id.unwrap_or(Value::Null),
            -32600,
            "Invalid JSON-RPC version",
        ));
    }

    // Notifications (no id) don't get responses
    let id = match request.id {
        Some(id) => id,
        None => {
            // Handle notification (no response needed)
            handle_notification(&request.method, request.params.as_ref());
            return None;
        }
    };

    let result = handle_method(&request.method, request.params.as_ref());

    match result {
        Ok(value) => Some(JsonRpcResponse::success(id, value)),
        Err(e) => Some(JsonRpcResponse::error(id, -32603, &e.to_string())),
    }
}

fn handle_notification(method: &str, _params: Option<&Value>) {
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

fn handle_method(method: &str, params: Option<&Value>) -> Result<Value> {
    match method {
        "initialize" => handle_initialize(params),
        "tools/list" => handle_tools_list(),
        "tools/call" => handle_tools_call(params),
        _ => anyhow::bail!("Method not found: {}", method),
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
                    "properties": {}
                }
            },
            {
                "name": "chant_status",
                "description": "Get project status summary with spec counts by status",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
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
                "name": "chant_resume",
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
        // Mutating tools
        "chant_spec_update" => tool_chant_spec_update(arguments),
        "chant_add" => tool_chant_add(arguments),
        "chant_finalize" => tool_chant_finalize(arguments),
        "chant_resume" => tool_chant_resume(arguments),
        "chant_cancel" => tool_chant_cancel(arguments),
        "chant_archive" => tool_chant_archive(arguments),
        _ => anyhow::bail!("Unknown tool: {}", name),
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
    let specs_dir = PathBuf::from(SPECS_DIR);
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
    specs.sort_by(|a, b| a.id.cmp(&b.id));

    // Filter by status if provided
    if let Some(args) = arguments {
        if let Some(status_str) = args.get("status").and_then(|v| v.as_str()) {
            let filter_status = match status_str {
                "pending" => Some(SpecStatus::Pending),
                "in_progress" => Some(SpecStatus::InProgress),
                "completed" => Some(SpecStatus::Completed),
                "failed" => Some(SpecStatus::Failed),
                _ => None,
            };

            if let Some(status) = filter_status {
                specs.retain(|s| s.frontmatter.status == status);
            }
        }
    }

    let specs_json: Vec<Value> = specs
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

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&specs_json)?
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

    let mut updated = false;

    // Update status if provided
    if let Some(status_str) = args.get("status").and_then(|v| v.as_str()) {
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
        spec.frontmatter.status = new_status;
        updated = true;
    }

    // Append output if provided
    if let Some(output) = args.get("output").and_then(|v| v.as_str()) {
        if !output.is_empty() {
            if !spec.body.ends_with('\n') && !spec.body.is_empty() {
                spec.body.push('\n');
            }
            spec.body.push_str("\n## Output\n\n");
            spec.body.push_str(output);
            spec.body.push('\n');
            updated = true;
        }
    }

    if !updated {
        return Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": "No updates specified. Provide 'status' or 'output' parameter."
                }
            ],
            "isError": true
        }));
    }

    // Save the spec
    let spec_path = specs_dir.join(format!("{}.md", spec.id));
    spec.save(&spec_path)?;

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": format!("Updated spec: {}", spec.id)
            }
        ]
    }))
}

// ============================================================================
// New Query Tools (read-only)
// ============================================================================

fn tool_chant_ready(_arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let mut specs = load_all_specs(&specs_dir)?;

    // Filter to ready specs (pending with no unmet dependencies)
    specs.retain(|s| {
        if s.frontmatter.status != SpecStatus::Pending {
            return false;
        }
        // Check if all dependencies are completed
        if let Some(deps) = &s.frontmatter.depends_on {
            let all_specs = load_all_specs(&specs_dir).unwrap_or_default();
            for dep_id in deps {
                let dep_completed = all_specs
                    .iter()
                    .any(|ds| ds.id == *dep_id && ds.frontmatter.status == SpecStatus::Completed);
                if !dep_completed {
                    return false;
                }
            }
        }
        true
    });

    specs.sort_by(|a, b| a.id.cmp(&b.id));

    let specs_json: Vec<Value> = specs
        .iter()
        .map(|s| {
            json!({
                "id": s.id,
                "title": s.title,
                "type": s.frontmatter.r#type,
                "labels": s.frontmatter.labels
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

fn tool_chant_status(_arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let specs = load_all_specs(&specs_dir)?;

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
            SpecStatus::Completed => completed += 1,
            SpecStatus::Failed => failed += 1,
            SpecStatus::Ready => pending += 1, // Ready is computed, treat as pending
            SpecStatus::Blocked => blocked += 1,
            SpecStatus::Cancelled => cancelled += 1,
            SpecStatus::NeedsAttention => needs_attention += 1,
        }
    }

    let status_json = json!({
        "total": specs.len(),
        "pending": pending,
        "in_progress": in_progress,
        "completed": completed,
        "failed": failed,
        "blocked": blocked,
        "cancelled": cancelled,
        "needs_attention": needs_attention
    });

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

    let lines = args.get("lines").and_then(|v| v.as_u64()).unwrap_or(100) as usize;

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
    let all_lines: Vec<&str> = content.lines().collect();

    // Return last N lines
    let start = if all_lines.len() > lines {
        all_lines.len() - lines
    } else {
        0
    };
    let log_output = all_lines[start..].join("\n");

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": log_output
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

    specs.sort_by(|a, b| a.id.cmp(&b.id));

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

// ============================================================================
// New Mutating Tools
// ============================================================================

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

    // Generate ID
    let new_id = id::generate_id(&specs_dir)?;
    let filename = format!("{}.md", new_id);
    let filepath = specs_dir.join(&filename);

    // Create spec content
    let prompt_line = match prompt {
        Some(p) => format!("prompt: {}\n", p),
        None => String::new(),
    };

    let content = format!(
        r#"---
type: code
status: pending
{}---

# {}
"#,
        prompt_line, description
    );

    std::fs::write(&filepath, content)?;

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": format!("Created spec: {}", new_id)
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

    // Update status to completed
    spec.frontmatter.status = SpecStatus::Completed;
    spec.frontmatter.completed_at = Some(chrono::Local::now().to_rfc3339());

    // Save the spec
    let spec_path = specs_dir.join(format!("{}.md", spec.id));
    spec.save(&spec_path)?;

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": format!("Finalized spec: {}", spec_id)
            }
        ]
    }))
}

fn tool_chant_resume(arguments: Option<&Value>) -> Result<Value> {
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

    // Check if spec is in failed or in_progress state
    if spec.frontmatter.status != SpecStatus::Failed
        && spec.frontmatter.status != SpecStatus::InProgress
    {
        return Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": format!("Spec '{}' is not in failed or in_progress state (current: {:?}). Only failed or in_progress specs can be resumed.", spec_id, spec.frontmatter.status)
                }
            ],
            "isError": true
        }));
    }

    // Reset to pending
    spec.frontmatter.status = SpecStatus::Pending;

    // Save the spec
    let spec_path = specs_dir.join(format!("{}.md", spec.id));
    spec.save(&spec_path)?;

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": format!("Resumed spec '{}' - reset to pending", spec_id)
            }
        ]
    }))
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

    // Check if already cancelled
    if spec.frontmatter.status == SpecStatus::Cancelled {
        return Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": format!("Spec '{}' is already cancelled", spec_id)
                }
            ],
            "isError": true
        }));
    }

    // Set status to cancelled
    spec.frontmatter.status = SpecStatus::Cancelled;

    // Save the spec
    let spec_path = specs_dir.join(format!("{}.md", spec.id));
    spec.save(&spec_path)?;

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": format!("Cancelled spec: {}", spec_id)
            }
        ]
    }))
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

    // Check if completed
    if spec.frontmatter.status != SpecStatus::Completed {
        return Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": format!("Spec '{}' must be completed to archive (current: {:?})", spec_id, spec.frontmatter.status)
                }
            ],
            "isError": true
        }));
    }

    let archive_dir = PathBuf::from(ARCHIVE_DIR);

    // Create archive directory if it doesn't exist
    std::fs::create_dir_all(&archive_dir)?;

    let source_path = specs_dir.join(format!("{}.md", spec_id));
    let dest_path = archive_dir.join(format!("{}.md", spec_id));

    // Move the spec file
    std::fs::rename(&source_path, &dest_path)?;

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": format!("Archived spec: {} -> {}", spec_id, dest_path.display())
            }
        ]
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_initialize() {
        let result = handle_initialize(None).unwrap();
        assert_eq!(result["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(result["serverInfo"]["name"], SERVER_NAME);
    }

    #[test]
    fn test_handle_tools_list() {
        let result = handle_tools_list().unwrap();
        let tools = result["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 13);
        // Query tools (7)
        assert_eq!(tools[0]["name"], "chant_spec_list");
        assert_eq!(tools[1]["name"], "chant_spec_get");
        assert_eq!(tools[2]["name"], "chant_ready");
        assert_eq!(tools[3]["name"], "chant_status");
        assert_eq!(tools[4]["name"], "chant_log");
        assert_eq!(tools[5]["name"], "chant_search");
        assert_eq!(tools[6]["name"], "chant_diagnose");
        // Mutating tools (6)
        assert_eq!(tools[7]["name"], "chant_spec_update");
        assert_eq!(tools[8]["name"], "chant_add");
        assert_eq!(tools[9]["name"], "chant_finalize");
        assert_eq!(tools[10]["name"], "chant_resume");
        assert_eq!(tools[11]["name"], "chant_cancel");
        assert_eq!(tools[12]["name"], "chant_archive");
    }

    #[test]
    fn test_json_rpc_response_success() {
        let resp = JsonRpcResponse::success(json!(1), json!({"test": true}));
        assert_eq!(resp.jsonrpc, "2.0");
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_json_rpc_response_error() {
        let resp = JsonRpcResponse::error(json!(1), -32600, "Invalid request");
        assert_eq!(resp.jsonrpc, "2.0");
        assert!(resp.result.is_none());
        assert!(resp.error.is_some());
        assert_eq!(resp.error.as_ref().unwrap().code, -32600);
    }
}
